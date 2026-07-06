function f52(a, b) { let v0 = 3; let v1 = 3; let v2 = 2; for (let i = 0; i !== a; i = i + 1) { v2 = a * a; v1 = v0 + v0; v1 = a + v0; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f52(k % 7, k);
console.log(acc);
console.log(f52(5, 3));
console.log(f52(20, 2147483));
console.log(f52(3, 0.5));
console.log(f52(4, -7.25));
console.log(f52(12, 1000000.0));
console.log(f52(0, 9));
console.log(f52(30, 65536));
