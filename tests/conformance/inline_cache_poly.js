function getX(o) { return o.x; }
const shapes = [{ x: 1 }, { y: 2, x: 2 }, { z: 3, y: 3, x: 3 }, { w: 4, x: 4 }, { v: 5, x: 5 }, { u: 6, x: 6 }];
let s = 0;
for (let r = 0; r < 100; r++) for (const o of shapes) s += getX(o);
console.log(s);
function setY(o, v) { o.y = v; return o; }
console.log(setY({}, 1), setY({ a: 1 }, 2), setY({ y: 0 }, 3));
class P { constructor(a) { this.a = a; } m() { return this.a; } }
let t = 0; for (let i = 0; i < 1000; i++) t += new P(i).m(); console.log(t);
