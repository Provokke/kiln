function f0(a, b) { let v0 = 2; let v1 = 1; for (let i = 0; i <= a; i = i + 1) { v1 = v1 - a; if (v1 > 1867) v1 = v1 % 89; v1 = v1 - b; v0 = v0 + v1; if (v0 > 62314) v0 = v0 % 440; v0 = a + v0; if (v0 > 56235) v0 = v0 % 704; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f0(k % 7, k);
console.log(acc);
console.log(f0(5, 3));
console.log(f0(20, 2147483));
console.log(f0(3, 0.5));
console.log(f0(4, -7.25));
console.log(f0(12, 1000000.0));
console.log(f0(0, 9));
console.log(f0(30, 65536));
