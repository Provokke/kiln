class Vec { constructor(x, y) { this.x = x; this.y = y; } add(o) { return new Vec(this.x + o.x, this.y + o.y); } len2() { return this.x * this.x + this.y * this.y; } }
let acc = new Vec(0, 0); let s = 0;
for (let i = 0; i < 300000; i++) { acc = acc.add(new Vec(i % 7, i % 3)); s = (s + acc.len2()) % 1000003; }
const arr = []; for (let i = 0; i < 100000; i++) arr.push({ id: i, v: i * 2 });
console.log(s, arr.filter(o => o.v % 3 === 0).map(o => o.id).reduce((a, b) => a + b, 0));
