function f40(a, b) { let v0 = 2; let v1 = 4; let v2 = 1; for (let i = 0; i < a; i = i + 1) { v1 = v1 * v1; v2 = v1 + v0; v2 = a + v2; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f40(k % 7, k);
console.log(acc);
console.log(f40(5, 3));
console.log(f40(20, 2147483));
console.log(f40(3, 0.5));
console.log(f40(4, -7.25));
console.log(f40(12, 1000000.0));
console.log(f40(0, 9));
console.log(f40(30, 65536));
