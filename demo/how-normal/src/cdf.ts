export interface CDF {
  p(x: number): number;
  dx(x: number): number;
  toHTML(): string;
}
