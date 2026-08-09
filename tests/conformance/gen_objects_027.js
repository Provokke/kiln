const o = { a: null, e: null, value: 12, f: 'gamma', c: true };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.c = 1; o.extra = 'x'; console.log(o.c, o.extra, Object.keys(o).length);
const c = { ...o, c: 'over' }; console.log(c.c, JSON.stringify(c));
class K27 {
  constructor(v) { this.v = v; this.c = v * 2; }
  get27() { return this.v + this.c; }
}
class L27 extends K27 { get27() { return super.get27() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K27(j) : new L27(j));
console.log(arr.map(x => x.get27()), arr[1], arr[0] instanceof K27);
