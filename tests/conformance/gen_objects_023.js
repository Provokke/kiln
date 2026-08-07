const o = { e: 'fire', a: 'x', id: null, c: 'clay', f: null };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.id = 4; o.extra = 'x'; console.log(o.id, o.extra, Object.keys(o).length);
const c = { ...o, id: 'over' }; console.log(c.id, JSON.stringify(c));
class K23 {
  constructor(v) { this.v = v; this.id = v * 2; }
  get23() { return this.v + this.id; }
}
class L23 extends K23 { get23() { return super.get23() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K23(j) : new L23(j));
console.log(arr.map(x => x.get23()), arr[1], arr[0] instanceof K23);
