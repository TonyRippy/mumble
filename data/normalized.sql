CREATE TABLE IF NOT EXISTS [label_set] (
   id INTEGER PRIMARY KEY AUTOINCREMENT,
   labels TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS [cluster_group] (
   id INTEGER PRIMARY KEY AUTOINCREMENT,
   config TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS [cluster] (
   id INTEGER PRIMARY KEY AUTOINCREMENT,
   group_id INTEGER NOT NULL,
   centroid BLOB NOT NULL,

   FOREIGN KEY (group_id) REFERENCES [cluster_group] (id)
);

CREATE TABLE IF NOT EXISTS [monitoring_data] (
   timestamp DATETIME NOT NULL,
   label_set_id INTEGER NOT NULL,
   cluster_id INTEGER NOT NULL,
   count INTEGER NOT NULL,

   PRIMARY KEY (timestamp, label_set_id),

   FOREIGN KEY (label_set_id) REFERENCES [label_set] (id),
   FOREIGN KEY (cluster_id) REFERENCES [cluster] (id)
);
