function f49(a, b) { let v0 = 3; let v1 = 2; for (let i = 0; i !== a; i = i + 1) { v1 = a * v0; v1 = v0 - 6; v1 = b - b; v0 = 9 * b; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f49(k % 7, k);
console.log(acc);
console.log(f49(5, 3));
console.log(f49(20, 2147483));
console.log(f49(3, 0.5));
console.log(f49(4, -7.25));
console.log(f49(12, 1000000.0));
console.log(f49(0, 9));
console.log(f49(30, 65536));
