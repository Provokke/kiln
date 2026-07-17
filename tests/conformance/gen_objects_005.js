const o = { id: null, e: 80, c: [1, 2] };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.id = 6; o.extra = 'x'; console.log(o.id, o.extra, Object.keys(o).length);
const c = { ...o, id: 'over' }; console.log(c.id, JSON.stringify(c));
class K5 {
  constructor(v) { this.v = v; this.id = v * 2; }
  get5() { return this.v + this.id; }
}
class L5 extends K5 { get5() { return super.get5() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K5(j) : new L5(j));
console.log(arr.map(x => x.get5()), arr[1], arr[0] instanceof K5);
