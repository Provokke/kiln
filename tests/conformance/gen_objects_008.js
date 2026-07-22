const o = { f: [1, 2], e: true, name: true, c: null };
console.log(o);
console.log(JSON.stringify(o));
console.log(Object.keys(o));
o.c = 1; o.extra = 'x'; console.log(o.c, o.extra, Object.keys(o).length);
const c = { ...o, c: 'over' }; console.log(c.c, JSON.stringify(c));
class K8 {
  constructor(v) { this.v = v; this.c = v * 2; }
  get8() { return this.v + this.c; }
}
class L8 extends K8 { get8() { return super.get8() * 10; } }
const arr = [];
for (let j = 0; j < 5; j++) arr.push(j % 2 ? new K8(j) : new L8(j));
console.log(arr.map(x => x.get8()), arr[1], arr[0] instanceof K8);
