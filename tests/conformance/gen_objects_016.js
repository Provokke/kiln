const o = { d: 5, value: [1, 2] };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.value = 5; o.extra = 'x'; console.log(o.value, o.extra, Object.keys(o).length);
const c = { ...o, value: 'over' }; console.log(c.value, JSON.stringify(c));
class K16 {
  constructor(v) { this.v = v; this.value = v * 2; }
  get16() { return this.v + this.value; }
}
class L16 extends K16 { get16() { return super.get16() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K16(j) : new L16(j));
console.log(arr.map(x => x.get16()), arr[1], arr[0] instanceof K16);
