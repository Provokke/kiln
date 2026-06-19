function f22(a, b) { let v0 = 2; let v1 = 3; let v2 = 0; for (let i = 0; i <= a; i = i + 1) { v1 = v1 + a; v2 = 3 * v0; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f22(k % 7, k);
console.log(acc);
console.log(f22(5, 3));
console.log(f22(20, 2147483));
console.log(f22(3, 0.5));
console.log(f22(4, -7.25));
console.log(f22(12, 1000000.0));
console.log(f22(0, 9));
console.log(f22(30, 65536));
