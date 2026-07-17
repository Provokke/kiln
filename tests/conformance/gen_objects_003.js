const o = { b: 'beta', value: null };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.value = 8; o.extra = 'x'; console.log(o.value, o.extra, Object.keys(o).length);
const c = { ...o, value: 'over' }; console.log(c.value, JSON.stringify(c));
class K3 {
  constructor(v) { this.v = v; this.value = v * 2; }
  get3() { return this.v + this.value; }
}
class L3 extends K3 { get3() { return super.get3() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K3(j) : new L3(j));
console.log(arr.map(x => x.get3()), arr[1], arr[0] instanceof K3);
