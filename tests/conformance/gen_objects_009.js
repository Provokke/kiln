const o = { id: true, b: [1, 2], f: [1, 2], a: 82 };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.a = 9; o.extra = 'x'; console.log(o.a, o.extra, Object.keys(o).length);
const c = { ...o, a: 'over' }; console.log(c.a, JSON.stringify(c));
class K9 {
  constructor(v) { this.v = v; this.a = v * 2; }
  get9() { return this.v + this.a; }
}
class L9 extends K9 { get9() { return super.get9() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K9(j) : new L9(j));
console.log(arr.map(x => x.get9()), arr[1], arr[0] instanceof K9);
