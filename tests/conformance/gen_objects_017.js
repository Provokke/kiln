const o = { a: 14, id: '', f: 'glaze', d: true };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.a = 6; o.extra = 'x'; console.log(o.a, o.extra, Object.keys(o).length);
const c = { ...o, a: 'over' }; console.log(c.a, JSON.stringify(c));
class K17 {
  constructor(v) { this.v = v; this.a = v * 2; }
  get17() { return this.v + this.a; }
}
class L17 extends K17 { get17() { return super.get17() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K17(j) : new L17(j));
console.log(arr.map(x => x.get17()), arr[1], arr[0] instanceof K17);
