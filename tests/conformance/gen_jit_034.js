function f34(a, b) { let v0 = 1; let v1 = 0; let v2 = 5; for (let i = 0; i <= a; i = i + 1) { v1 = 5 * v2; v1 = v1 * v1; if (v1 > 98748) v1 = v1 % 278; v2 = b - v1; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f34(k % 7, k);
console.log(acc);
console.log(f34(5, 3));
console.log(f34(20, 2147483));
console.log(f34(3, 0.5));
console.log(f34(4, -7.25));
console.log(f34(12, 1000000.0));
console.log(f34(0, 9));
console.log(f34(30, 65536));
