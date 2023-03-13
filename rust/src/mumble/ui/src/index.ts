// Mumble Client-Side UI
// Copyright (C) 2023, Tony Rippy
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE file at the
// root of this repository, or online at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import * as ecdfs from 'ecdfs'

let graph = ecdfs.createGraph('cdf')

interface TargetData {
}

interface MetricData {
  id: number;
  name: string;

}

interface UpdateData {
  id: any;
  ecdf: Array<[number, number]>;
}

class Metric {
  minX: number;
  maxX: number;

  constructor(public id: number) {
  }

  onUpdate(ecdf: ecdfs.ECDF) {
    document.getElementById('cdf-text').innerText = JSON.stringify(ecdf);

    // Plot the distributions:
    let p = ecdf.getRawCDF();

    // Size the graph
    const xMargin = (p.maxX - p.minX) * 0.1
    const minX = p.minX - xMargin;
    if (this.minX === undefined || minX < this.minX) this.minX = minX
    const maxX = p.maxX + xMargin;
    if (this.maxX === undefined || maxX > this.maxX) this.maxX = maxX
    graph.setRangeX(this.minX, this.maxX)

    // Render this as an image.
    graph.layers[0] = new ecdfs.Layer(
      ecdfs.getRenderFuncForPlot(p),
      { // line
        r: 0,
        g: 0,
        b: 255,
        a: 255
      },
      { // fill
        r: 0,
        g: 0,
        b: 255,
        a: 128,
      });
    graph.draw()
  }
}

class MonitoringTarget {
  metrics: Map<any, Metric>;

  constructor(data: TargetData) {
    this.metrics = new Map<any, Metric>();
  }

  onUpdate(data: UpdateData) {
    let metric = this.metrics.get(data.id);
    if (metric === undefined) {
      metric = new Metric(data.id)
      this.metrics.set(data.id, metric)
    }
    metric.onUpdate(ecdfs.fromJSON(data.ecdf))
  }
}

var target: MonitoringTarget = null;
var eventSource = new EventSource('/push');

eventSource.onerror = (e) => {
  console.error("An error occurred when trying to connect to the target's push service.");
};

eventSource.onopen = (e) => {
  console.info("A connection to the target's push service has been established.");
  window.addEventListener('unload', (e) => {
    eventSource.close();
  });
};

eventSource.addEventListener('target', (e: MessageEvent) => {
  console.debug('Got "target" event: ', e.lastEventId);
  console.assert(target == null, "Target already set!");
  let data = <TargetData>JSON.parse(e.data);
  target = new MonitoringTarget(data);
});

eventSource.addEventListener('update', (e: MessageEvent) => {
  console.debug('Got "update" event: ', e.lastEventId);
  if (target == null) {
    console.error("Target not initialized yet!");
  } else {
    let data = <UpdateData>JSON.parse(e.data);
    target.onUpdate(data);
  }
});
