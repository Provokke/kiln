function f23(a, b) { let v0 = 0; let v1 = 4; for (let i = 0; i < a; i = i + 1) { v0 = a - a; v1 = v0 - v0; if (v1 > 68851) v1 = v1 % 721; v0 = v1 - v0; v0 = 8 * v0; v1 = v0 * v0; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f23(k % 7, k);
console.log(acc);
console.log(f23(5, 3));
console.log(f23(20, 2147483));
console.log(f23(3, 0.5));
console.log(f23(4, -7.25));
console.log(f23(12, 1000000.0));
console.log(f23(0, 9));
console.log(f23(30, 65536));
