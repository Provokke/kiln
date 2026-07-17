const o = { e: null, id: 'beta', c: true, b: 98, value: null };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.c = 9; o.extra = 'x'; console.log(o.c, o.extra, Object.keys(o).length);
const c = { ...o, c: 'over' }; console.log(c.c, JSON.stringify(c));
class K4 {
  constructor(v) { this.v = v; this.c = v * 2; }
  get4() { return this.v + this.c; }
}
class L4 extends K4 { get4() { return super.get4() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K4(j) : new L4(j));
console.log(arr.map(x => x.get4()), arr[1], arr[0] instanceof K4);
