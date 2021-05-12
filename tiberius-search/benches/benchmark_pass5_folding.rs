
use tiberius_search::parse;
use criterion::{criterion_group, criterion_main, Criterion, Throughput,BenchmarkId};

criterion_group!(benches, bench_query_sampler);
criterion_main!(benches);

fn bench_query_sampler(b: &mut Criterion) {
    let input = vec![
        "sg",
        "sg AND (-pony-,(:),human (eqg)))",
        "sg,safe,pony,cute,score.gte:100",
        "sg,safe,pony,cute,score.gte:100,attribute.lt:20.035",
        "pride flag, -oc, -twilight sparkle, -fluttershy, -pinkie pie, -rainbow dash, -applejack, -rarity",
        "pony OR human",
        "created_at:2015-04 01:00:50Z",
        "created_at.gte:3 days ago",
        "time\\,space",
        "artist:bibi_8_8_ AND eqg human",
        "species:eqg human",
        "sglong",
        "-(species:pony || species:eqg human&&pony)",
        "sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg,sg",
        "species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human,species:eqg human",
        "((((((((((((((((((((((((((((((((((boop)))))))))))))))))))))))))))))))))))",
    ];
    let mut group = b.benchmark_group("from_str");
    group.noise_threshold(0.10f64);
    for (id, query) in input.into_iter().enumerate() {
        group.throughput(Throughput::Elements(query.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(id), query, |b, query| {
            b.iter(|| {
                parse(query.to_string())
            })
        });
    }
    group.finish();
}