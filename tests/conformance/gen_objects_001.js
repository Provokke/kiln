const o = { id: 40, c: true };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.id = 0; o.extra = 'x'; console.log(o.id, o.extra, Object.keys(o).length);
const c = { ...o, id: 'over' }; console.log(c.id, JSON.stringify(c));
class K1 {
  constructor(v) { this.v = v; this.id = v * 2; }
  get1() { return this.v + this.id; }
}
class L1 extends K1 { get1() { return super.get1() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K1(j) : new L1(j));
console.log(arr.map(x => x.get1()), arr[1], arr[0] instanceof K1);
