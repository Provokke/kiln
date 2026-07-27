const o = { value: null, id: 0, d: null };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.value = 7; o.extra = 'x'; console.log(o.value, o.extra, Object.keys(o).length);
const c = { ...o, value: 'over' }; console.log(c.value, JSON.stringify(c));
class K12 {
  constructor(v) { this.v = v; this.value = v * 2; }
  get12() { return this.v + this.value; }
}
class L12 extends K12 { get12() { return super.get12() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K12(j) : new L12(j));
console.log(arr.map(x => x.get12()), arr[1], arr[0] instanceof K12);
