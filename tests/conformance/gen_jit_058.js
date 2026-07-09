function f58(a, b) { let v0 = 2; for (let i = 0; i < a; i = i + 1) { v0 = 6 - b; v0 = v0 * b; if (v0 > 71323) v0 = v0 % 965; v0 = b * b; v0 = b - a; if (v0 > 34237) v0 = v0 % 588; v0 = a + b; } return v0; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f58(k % 7, k);
console.log(acc);
console.log(f58(5, 3));
console.log(f58(20, 2147483));
console.log(f58(3, 0.5));
console.log(f58(4, -7.25));
console.log(f58(12, 1000000.0));
console.log(f58(0, 9));
console.log(f58(30, 65536));
