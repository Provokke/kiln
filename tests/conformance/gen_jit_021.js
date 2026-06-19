function f21(a, b) { let v0 = 2; for (let i = 0; i !== a; i = i + 1) { v0 = a - a; v0 = v0 - a; if (v0 > 28157) v0 = v0 % 394; v0 = b - 8; v0 = b - a; } return v0; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f21(k % 7, k);
console.log(acc);
console.log(f21(5, 3));
console.log(f21(20, 2147483));
console.log(f21(3, 0.5));
console.log(f21(4, -7.25));
console.log(f21(12, 1000000.0));
console.log(f21(0, 9));
console.log(f21(30, 65536));
