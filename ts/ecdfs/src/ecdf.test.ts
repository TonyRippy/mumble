import { ECDF, fromJSON, toJSON, merge } from './ecdf';

import * as mocha from 'mocha';
import * as chai from 'chai';

const expect = chai.expect;

describe('ECDF', () => {

  it('should be empty at the start' , () => {
    let ecdf = new ECDF(); 
    expect(ecdf.x).to.be.empty;
    expect(ecdf.h).to.be.empty;
    expect(ecdf.n).to.equal(0);
  });

  it('should be able to add samples in any order' , () => {
    let ecdf = new ECDF();
    ecdf.addSample(2);
    ecdf.addSample(1);
    ecdf.addSample(4);
    ecdf.addSample(2);
    expect(ecdf.x).to.have.ordered.members([1, 2, 4]);
    expect(ecdf.h).to.have.ordered.members([1, 2, 1]);
    expect(ecdf.n).to.equal(3);
  });

  it('can be created from an empty JSON array' , () => {
    let ecdf = fromJSON([]);
    expect(ecdf.x).to.be.empty;
    expect(ecdf.h).to.be.empty;
    expect(ecdf.n).to.equal(0);
  });

  it('can be created from a JSON array' , () => {
    let ecdf = fromJSON([[1,2], [3,1], [5,3]]);
    expect(ecdf.x).to.have.ordered.members([1, 3, 5]);
    expect(ecdf.h).to.have.ordered.members([2, 1, 3]);
    expect(ecdf.n).to.equal(3);
  });

  it('can be serialized as a JSON array' , () => {
    let ecdf = new ECDF();
    ecdf.addSample(3);
    ecdf.addSample(2);
    ecdf.addSample(1);
    ecdf.addSample(0);
    ecdf.addSample(2);
    expect(toJSON(ecdf)).to.be.deep.equal(
      [[0, 1], [1, 1], [2, 2], [3, 1]]);
  });

  it('can be merged together' , () => {
    let a = new ECDF();
    let b = new ECDF();
    expect(merge(a, b)).to.be.deep.equal(a);

    a = fromJSON([[1, 1], [2, 1]]);
    b = new ECDF();
    expect(merge(a, b)).to.be.deep.equal(a);
    expect(merge(b, a)).to.be.deep.equal(a);

    a = fromJSON([[1, 1], [3, 1], [5, 2]]);
    b = fromJSON([[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [5,6]]);
    expect(merge(a, b)).to.be.deep.equal(
      fromJSON(
        [[0, 1], [1, 3], [2, 3], [3, 5], [4, 5], [5, 8]]));
  });

  // Problem case
  // x = [1, 1.5, 1.6, 2]
  // h = [1, 1,   1,   3]

});
