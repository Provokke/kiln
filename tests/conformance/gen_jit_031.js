function f31(a, b) { let v0 = 3; let v1 = 1; for (let i = 0; i < a; i = i + 1) { v1 = v0 + v1; v1 = v0 * v0; v0 = b + 5; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f31(k % 7, k);
console.log(acc);
console.log(f31(5, 3));
console.log(f31(20, 2147483));
console.log(f31(3, 0.5));
console.log(f31(4, -7.25));
console.log(f31(12, 1000000.0));
console.log(f31(0, 9));
console.log(f31(30, 65536));
