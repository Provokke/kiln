function f4(a, b) { let v0 = 0; let v1 = 4; let v2 = 0; for (let i = 0; i !== a; i = i + 1) { v2 = a * 8; v2 = v2 * a; v0 = b * b; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f4(k % 7, k);
console.log(acc);
console.log(f4(5, 3));
console.log(f4(20, 2147483));
console.log(f4(3, 0.5));
console.log(f4(4, -7.25));
console.log(f4(12, 1000000.0));
console.log(f4(0, 9));
console.log(f4(30, 65536));
