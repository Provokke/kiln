const o = { e: [1, 2], name: null };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.e = 3; o.extra = 'x'; console.log(o.e, o.extra, Object.keys(o).length);
const c = { ...o, e: 'over' }; console.log(c.e, JSON.stringify(c));
class K19 {
  constructor(v) { this.v = v; this.e = v * 2; }
  get19() { return this.v + this.e; }
}
class L19 extends K19 { get19() { return super.get19() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K19(j) : new L19(j));
console.log(arr.map(x => x.get19()), arr[1], arr[0] instanceof K19);
