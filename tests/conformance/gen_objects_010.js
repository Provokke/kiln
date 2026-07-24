const o = { c: '', e: 'gamma' };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.c = 5; o.extra = 'x'; console.log(o.c, o.extra, Object.keys(o).length);
const c = { ...o, c: 'over' }; console.log(c.c, JSON.stringify(c));
class K10 {
  constructor(v) { this.v = v; this.c = v * 2; }
  get10() { return this.v + this.c; }
}
class L10 extends K10 { get10() { return super.get10() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K10(j) : new L10(j));
console.log(arr.map(x => x.get10()), arr[1], arr[0] instanceof K10);
