CREATE TABLE [label_set] (
   id INTEGER PRIMARY KEY AUTOINCREMENT,
   labels TEXT NOT NULL
);

CREATE TABLE [monitoring_data] (
   timestamp DATETIME NOT NULL,
   label_set_id INTEGER NOT NULL,
   data BLOB NOT NULL,

   PRIMARY KEY (timestamp, label_set_id),

   FOREIGN KEY (label_set_id) REFERENCES [label_set] (id) 
     ON DELETE CASCADE ON UPDATE NO ACTION
);
