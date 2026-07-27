const o = { name: 'kiln', d: true, a: 42, b: [1, 2] };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.name = 2; o.extra = 'x'; console.log(o.name, o.extra, Object.keys(o).length);
const c = { ...o, name: 'over' }; console.log(c.name, JSON.stringify(c));
class K14 {
  constructor(v) { this.v = v; this.name = v * 2; }
  get14() { return this.v + this.name; }
}
class L14 extends K14 { get14() { return super.get14() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K14(j) : new L14(j));
console.log(arr.map(x => x.get14()), arr[1], arr[0] instanceof K14);
