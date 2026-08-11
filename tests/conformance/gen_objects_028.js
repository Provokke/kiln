const o = { c: 'gamma', name: 'clay', d: 'delta', id: 'kiln' };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.name = 7; o.extra = 'x'; console.log(o.name, o.extra, Object.keys(o).length);
const c = { ...o, name: 'over' }; console.log(c.name, JSON.stringify(c));
class K28 {
  constructor(v) { this.v = v; this.name = v * 2; }
  get28() { return this.v + this.name; }
}
class L28 extends K28 { get28() { return super.get28() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K28(j) : new L28(j));
console.log(arr.map(x => x.get28()), arr[1], arr[0] instanceof K28);
