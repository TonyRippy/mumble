use flame_clustering::{DistanceGraph, ObjectType};
use mumble::ecdf::ECDF;
use std::{collections::HashSet, io};

fn main() {
    let ecdfs: Vec<ECDF<f64>> = io::stdin()
        .lines()
        .map(|x| {
            let ecdf: ECDF<f64> = serde_json::from_str(&x.unwrap()).unwrap();
            ecdf
        })
        .collect();

    let graph = DistanceGraph::build(&ecdfs, |a, b| a.area_difference(b));
    let csos = graph
        .find_supporting_objects(3, -1.0)
        .approximate_fuzzy_memberships(100, 1e-6);

    let (clusters, outliers) = csos.make_clusters(-1.0);

    println!("graph {{");
    for (cid, cluster) in clusters.iter().enumerate() {
        println!("  subgraph cluster_{} {{", cid);
        for &id in cluster {
            print!("    n{} [label=\"{}\"", id, id);
            match csos.object_type(id) {
                ObjectType::Support => {
                    print!(" color=\"blue\" style=\"bold\"");
                }
                ObjectType::Outlier => {
                    print!(" color=\"red\"");
                }
                _ => {}
            }
            println!("];");
        }
        println!("    label=\"cluster {}\";", cid);
        println!("    graph[style=solid];");
        println!("  }}");
    }
    for id in outliers {
        println!("  n{} [label=\"{}\"];", id, id);
    }
    let mut edges = HashSet::new();
    for id in 0..ecdfs.len() {
        for (n, d) in graph.neighbors(id) {
            let key = if id < n { (id, n) } else { (n, id) };
            if !edges.contains(&key) {
                println!(
                    "  n{} -- n{} [style=dashed tooltip=\"{}\" len={}];",
                    id, n, d, d
                );
                edges.insert(key);
            }
        }
    }
    println!("}}");
}
