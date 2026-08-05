const o = { name: 42, id: '', d: [1, 2], a: 'fire', f: [1, 2] };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.f = 6; o.extra = 'x'; console.log(o.f, o.extra, Object.keys(o).length);
const c = { ...o, f: 'over' }; console.log(c.f, JSON.stringify(c));
class K22 {
  constructor(v) { this.v = v; this.f = v * 2; }
  get22() { return this.v + this.f; }
}
class L22 extends K22 { get22() { return super.get22() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K22(j) : new L22(j));
console.log(arr.map(x => x.get22()), arr[1], arr[0] instanceof K22);
