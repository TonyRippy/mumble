export class Point {
  constructor(public x: number, public y: number) {
  }

  lessThan(other: Point): boolean {
    return this.x < other.x || (this.x == other.x && this.y < other.y);
  }

  equalTo(other: Point): boolean {
    return this.x == other.x && this.y == other.y;
  }
}
