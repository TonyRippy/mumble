use crate::Id;

use mumble::ecdf::InterpolatedECDF;

// TODO: Support different cluster groups

pub struct DataStore {
    cluster_group: ClusterGroup,
    cluster_max: usize,
    connection: sqlite::Connection,
}

impl DataStore {
    pub fn open(database: &str, eps: f64) -> sqlite::Result<DataStore> {
        Ok(DataStore {
            cluster_group: ClusterGroup::new(eps),
            cluster_max: 0,
            connection: sqlite::open(database)?,
        })
    }

    fn write_cluster(
        &self,
        id: usize,
        centroid: &InterpolatedECDF<f64>,
        eps: f64,
    ) -> sqlite::Result<()> {
        let rmp = rmp_serde::to_vec(centroid).expect("serialize centroid");
        let mut statement = self
            .connection
            .prepare("INSERT INTO cluster (id, group_id, centroid, eps) VALUES (?, 1, ?, ?)")?;
        statement.bind((1, id as i64))?;
        statement.bind((2, &rmp as &[u8]))?;
        statement.bind((3, eps))?;
        statement.next()?;
        Ok(())
    }

    fn write_sample(&self, id: Id, cluster_id: usize, count: usize) -> sqlite::Result<()> {
        let mut statement = self.connection.prepare(
            "INSERT INTO monitoring_data (timestamp, label_set_id, cluster_id, count) VALUES (?, ?, ?, ?)",
        ).expect("prepare insert");
        statement
            .bind((1, id.timestamp.as_str()))
            .expect("bind timestamp");
        // TODO: Copy label_sets into the output database
        statement.bind((2, 1)).expect("bind label_set_id");
        statement
            .bind((3, cluster_id as i64))
            .expect("bind cluster_id");
        statement.bind((4, count as i64)).expect("bind count");
        statement.next().expect("execute insert");
        Ok(())
    }

    pub fn process_batch(&mut self, batch: Vec<(Id, InterpolatedECDF<f64>)>) {
        let mut ids = Vec::with_capacity(batch.len());
        let mut ecdfs = Vec::with_capacity(batch.len());
        for (id, h) in batch.into_iter() {
            ids.push(id);
            ecdfs.push(h);
        }
        let assignments = self.cluster_group.process_batch(&ecdfs);
        assert_eq!(ids.len(), assignments.len());

        // Write out any new clusters
        let new_max = self.cluster_group.centroids.len();
        for cluster_id in self.cluster_max..new_max {
            let (centroid, eps) = &self.cluster_group.centroids[cluster_id];
            self.write_cluster(cluster_id, centroid, *eps)
                .expect("write cluster");
        }
        self.cluster_max = new_max;

        // Write out the samples
        for ((id, cluster_id), count) in ids
            .into_iter()
            .zip(assignments.into_iter())
            .zip(ecdfs.into_iter().map(|ecdf| ecdf.len().round() as usize))
        {
            self.write_sample(id, cluster_id, count)
                .expect("write sample");
        }
    }
}

/// Classification according to the DBSCAN algorithm
#[derive(Debug, Copy, Clone)]
pub enum Assignment {
    Unassigned,
    Assigned(usize),
}

impl Assignment {
    pub fn is_assigned(&self) -> bool {
        matches!(self, Assignment::Assigned(_))
    }
}

struct ClusterGroup {
    centroids: Vec<(InterpolatedECDF<f64>, f64)>,
    eps: f64,
}

impl ClusterGroup {
    pub fn new(eps: f64) -> ClusterGroup {
        ClusterGroup {
            eps,
            centroids: Vec::new(),
        }
    }

    fn find_neighbors<'a>(
        sample: &'a InterpolatedECDF<f64>,
        population: &'a [InterpolatedECDF<f64>],
        assignments: &'a [Assignment],
        eps: f64,
    ) -> impl Iterator<Item = usize> + 'a {
        population
            .iter()
            .enumerate()
            .filter(move |&(idx, pt)| {
                if assignments[idx].is_assigned() {
                    return false;
                }
                let distance = sample.area_difference(pt);
                distance < eps
            })
            .map(|(idx, _)| idx)
    }

    fn expand_cluster(
        queue: &mut Vec<usize>,
        population: &[InterpolatedECDF<f64>],
        assignments: &mut [Assignment],
        eps: f64,
        cluster: usize,
    ) -> bool {
        if queue.is_empty() {
            return false;
        }
        while let Some(idx) = queue.pop() {
            assignments[idx] = Assignment::Assigned(cluster);
            let neighbors = Self::find_neighbors(&population[idx], population, assignments, eps);
            queue.extend(neighbors);
        }
        true
    }

    /// Run a dumb version of DBSCAN on a set of samples.
    fn run(&mut self, samples: &[InterpolatedECDF<f64>]) -> Vec<Assignment> {
        let mut assignments = vec![Assignment::Unassigned; samples.len()];
        let mut neighbors = Vec::new();
        let mut cluster = 0;

        for (centroid, _) in self.centroids.iter() {
            // Seed the run with known clusters
            neighbors.clear();
            neighbors.extend(Self::find_neighbors(
                centroid,
                samples,
                &assignments,
                self.eps,
            ));
            for idx in neighbors.iter() {
                assignments[*idx] = Assignment::Assigned(cluster);
            }
            cluster += 1;
        }
        for idx in 0..samples.len() {
            // Scan all remaining samples and ensure they are assigned to new clusters
            if assignments[idx].is_assigned() {
                continue;
            }
            neighbors.clear();
            neighbors.extend(Self::find_neighbors(
                &samples[idx],
                samples,
                &assignments,
                self.eps,
            ));
            for idx in neighbors.iter() {
                assignments[*idx] = Assignment::Assigned(cluster);
            }
            cluster += 1;
        }
        assignments
    }

    fn report_clusters(
        &mut self,
        ecdfs: &Vec<InterpolatedECDF<f64>>,
        existing_clusters: Vec<(usize, Vec<usize>)>,
        new_clusters: Vec<Vec<usize>>,
    ) -> Vec<usize> {
        let mut cluster_mapping = vec![0usize; ecdfs.len()];

        for (cluster_id, cluster) in existing_clusters.into_iter() {
            debug!("Existing cluster {}: size +{}", cluster_id, cluster.len());
            for &j in cluster.iter() {
                cluster_mapping[j] = cluster_id;
            }
        }

        let offset = self.centroids.len();
        for new_cluster in new_clusters.iter() {
            let centroid = new_cluster
                .iter()
                .map(|&i| &ecdfs[i])
                .fold(InterpolatedECDF::default(), |acc, x| acc.merge(x));
            // let eps = if new_cluster.len() > 1 {
            //     new_cluster
            //         .iter()
            //         .map(|&i| centroid.area_difference(&ecdfs[i]))
            //         .reduce(f64::max)
            //         .unwrap()
            // } else {
            //     self.eps
            // };
            self.centroids.push((centroid, self.eps));
        }
        for (i, cluster) in new_clusters.into_iter().enumerate() {
            let cluster_id = i + offset;
            debug!("New cluster {}: size {}", cluster_id, cluster.len());
            for &j in cluster.iter() {
                cluster_mapping[j] = cluster_id;
            }
        }
        cluster_mapping
    }

    pub fn process_batch(&mut self, ecdfs: &Vec<InterpolatedECDF<f64>>) -> Vec<usize> {
        info!("Processing batch of {} samples... ", ecdfs.len());
        self.run(ecdfs);
        let mut cluster_map = self
            .run(ecdfs)
            .into_iter()
            .enumerate()
            .map(|(id, c)| match c {
                Assignment::Assigned(cluster) => (cluster, id),
                other => {
                    panic!("Unexpected classification: {:?}", other);
                }
            })
            .collect::<Vec<(usize, usize)>>();
        cluster_map.sort_unstable();

        let mut existing_clusters = Vec::new();
        let mut new_clusters = Vec::new();
        let mut cluster_ids = Vec::new();
        let mut last_cluster = cluster_map[0].0;
        for (cluster, id) in cluster_map {
            if cluster != last_cluster {
                if last_cluster < self.centroids.len() {
                    existing_clusters.push((last_cluster, cluster_ids));
                } else {
                    new_clusters.push(cluster_ids);
                }
                cluster_ids = Vec::new();
                last_cluster = cluster;
            }
            cluster_ids.push(id);
        }
        if last_cluster < self.centroids.len() {
            existing_clusters.push((last_cluster, cluster_ids));
        } else {
            new_clusters.push(cluster_ids);
        }
        self.report_clusters(ecdfs, existing_clusters, new_clusters)
    }
}
