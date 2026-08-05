const o = { f: true, c: 'glaze', b: true, e: 49 };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.e = 6; o.extra = 'x'; console.log(o.e, o.extra, Object.keys(o).length);
const c = { ...o, e: 'over' }; console.log(c.e, JSON.stringify(c));
class K20 {
  constructor(v) { this.v = v; this.e = v * 2; }
  get20() { return this.v + this.e; }
}
class L20 extends K20 { get20() { return super.get20() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K20(j) : new L20(j));
console.log(arr.map(x => x.get20()), arr[1], arr[0] instanceof K20);
