import { ECDF } from './ecdf';

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

  // Problem case
  // x = [1, 1.5, 1.6, 2]
  // h = [1, 1,   1,   3]

});
