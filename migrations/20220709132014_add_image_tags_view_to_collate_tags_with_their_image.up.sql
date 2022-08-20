-- Add up migration script here
create view image_tags
as
select
    i.id,
    array_agg(t.id) as tag_ids,
    array_agg(t."name") as tag_names,
    array_agg(t."namespace") as tag_namespaces,
    array_agg(t.name_in_namespace) as tag_name_in_namespaces,
    array_agg(t.category) as tag_categories,
    array_agg(t.slug) as tag_slugs,
    array_agg(t.description) as tag_descriptions,
    array_agg(t.images_count) as tag_images_counts
from images i
join image_taggings it on it.image_id = i.id 
join tags t on t.id = it.tag_id 
group by i.id;