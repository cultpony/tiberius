use axum::http::Uri;
use clap::ArgMatches;
use tiberius_core::app::DBPool;
use tiberius_core::config::Configuration;
use tiberius_core::error::TiberiusResult;
use tiberius_core::state::TiberiusState;
use tiberius_core::CSPHeader;
use tiberius_dependencies::casbin::{CoreApi, MgmtApi, RbacApi};
use tiberius_dependencies::{axum_csrf, axum_flash};

use crate::cli::GrantAclAction;

pub async fn grant_acl(
    args: &crate::cli::GrantAclCli,
    config: Configuration,
) -> TiberiusResult<()> {
    info!("Initializing Database connection");
    let db_conn: DBPool = config.db_conn().await?;
    let csrf_config = axum_csrf::CsrfConfig::default();
    let flash_key = axum_flash::Key::generate();
    let flash_config = axum_flash::Config::new(flash_key);
    let state = TiberiusState::new(
        config.clone(),
        tiberius_core::state::UrlDirections {
            login_page: Uri::default(),
        },
        csrf_config,
        flash_config,
        CSPHeader::default(),
    )
    .await?;
    let mut casbin = state.get_acl_enforcer().await?;
    let client = tiberius_models::Client::new(db_conn, config.search_dir.as_ref());
    let grant = args.act == GrantAclAction::Grant;
    let revoke = args.act == GrantAclAction::Revoke;
    let list = args.act == GrantAclAction::List;
    let user = args.user.clone().map(|x| format!("user::{}", x));
    let group = args.group.clone();
    let subject = args.subject.clone();
    let member_of = args.member_of.clone();
    let action = args.action.clone();
    assert!(
        !group
            .as_ref()
            .map(|x| x.starts_with("user::"))
            .unwrap_or(false),
        "Group cannot start with user prefix"
    );
    assert!(
        user.as_ref()
            .map(|x| x.starts_with("user::"))
            .unwrap_or(true),
        "User must start with user:: if present"
    );
    assert!(
        !member_of
            .as_ref()
            .map(|x| x.starts_with("user::"))
            .unwrap_or(false),
        "Member Of cannot start with user prefix"
    );
    assert!(!(grant && revoke), "Cannot grant & revoke at the same time");
    assert!(!(grant && list), "Cannot grant & list at the same time");
    assert!(!(list && revoke), "Cannot list & revoke at the same time");
    assert!(
        list || grant || revoke,
        "Atleast one subcommand must be set"
    );
    warn!("No DB Migrations are run, ensure your databse is up-to-date!");
    match (user.as_ref(), subject.as_ref(), action.as_ref()) {
        (Some(v), Some(w), Some(x)) => {
            if grant {
                casbin
                    .add_permission_for_user(v, vec![w.clone(), x.clone()])
                    .await?;
            } else if revoke {
                todo!("revoke ACL")
            } else if list {
                todo!("list ACL")
            } else {
                unreachable!();
            }
            return Ok(());
        }
        (Some(v), Some(w), None) => {
            todo!()
        }
        (Some(v), None, Some(x)) => {
            todo!()
        }
        v => {}
    }
    match (group.as_ref(), subject.as_ref(), action.as_ref()) {
        (Some(v), Some(w), Some(x)) => {
            todo!()
        }
        (Some(v), Some(w), None) => {
            todo!()
        }
        (Some(v), None, Some(x)) => {
            todo!()
        }
        _ => {}
    }
    match (user.as_ref(), member_of.as_ref()) {
        (Some(v), Some(w)) => {
            if grant {
                if casbin.has_role_for_user(v, w, None) {
                    warn!("ACL already present: {} -> {}", w, v);
                    return Ok(());
                }
                info!("Granting membership {} -> {}", w, v);
                casbin.add_role_for_user(v, w, None).await?;
            } else if revoke {
                if !casbin.has_role_for_user(v, w, None) {
                    warn!("ACL already present: {} -> {}", w, v);
                    return Ok(());
                }
                info!("Revoking membership {} -> {}", w, v);
                casbin.delete_role_for_user(v, w, None).await?;
            } else if list {
                error!("Cannot grant to user member-of");
            } else {
                unreachable!();
            }
            return Ok(());
        }
        (Some(v), None) => {
            if grant || revoke {
                error!("Cannot grant or revoke on user alone");
                return Ok(());
            }
            info!("Listing membership of {}", v);
            for role in casbin.get_implicit_roles_for_user(v, None) {
                println!("Role: {}", role);
            }
            return Ok(());
        }
        _ => {}
    }
    match (group.as_ref(), member_of.as_ref()) {
        (Some(v), Some(w)) => {
            if grant {
                if casbin.has_role_for_user(v, w, None) {
                    warn!("ACL already present: {} -> {}", w, v);
                    return Ok(());
                }
                info!("Granting membership {} -> {}", w, v);
                casbin.add_role_for_user(v, w, None).await?;
            } else if revoke {
                if !casbin.has_role_for_user(v, w, None) {
                    warn!("ACL already present: {} -> {}", w, v);
                    return Ok(());
                }
                info!("Revoking membership {} -> {}", w, v);
                casbin.delete_role_for_user(v, w, None).await?;
            } else if list {
                error!("Cannot grant to group member-of");
            } else {
                unreachable!();
            }
            return Ok(());
        }
        (Some(v), None) => {
            todo!()
        }
        _ => {}
    }
    warn!("Listing all ACL Entries, as no other option was given to filter output.");
    let roles = casbin.get_all_roles();
    for role in roles {
        let rm = casbin.get_role_manager();
        let rmr = rm.read();
        let users = rmr.get_users(&role, None);
        drop(rmr);
        drop(rm);
        for user in users {
            if user.starts_with("user::") {
                println!("User: {:?}", user);
            } else {
                println!("Role: {:?}", role);
            }
            println!("|\\ Direct");
            for role in casbin.get_roles_for_user(&user, None) {
                println!("| - Roles: {:?}", role)
            }
            for perm in casbin.get_permissions_for_user(&user, None) {
                println!("| - Permission: {:?}", perm)
            }
            println!("|\\ Implicit");
            for role in casbin.get_implicit_roles_for_user(&user, None) {
                println!("| - Roles: {:?}", role)
            }
            for perm in casbin.get_implicit_permissions_for_user(&user, None) {
                println!("| - Permission: {:?}", perm)
            }
        }
    }
    Ok(())
}
