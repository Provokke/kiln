const o = { value: 98, b: 'fire', c: 92 };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.c = 4; o.extra = 'x'; console.log(o.c, o.extra, Object.keys(o).length);
const c = { ...o, c: 'over' }; console.log(c.c, JSON.stringify(c));
class K26 {
  constructor(v) { this.v = v; this.c = v * 2; }
  get26() { return this.v + this.c; }
}
class L26 extends K26 { get26() { return super.get26() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K26(j) : new L26(j));
console.log(arr.map(x => x.get26()), arr[1], arr[0] instanceof K26);
