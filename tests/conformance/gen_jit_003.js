function f3(a, b) { let v0 = 4; for (let i = 0; i <= a; i = i + 1) { v0 = 3 - b; if (v0 > 29518) v0 = v0 % 108; v0 = a + v0; if (v0 > 25488) v0 = v0 % 160; v0 = b * a; v0 = 6 - v0; } return v0; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f3(k % 7, k);
console.log(acc);
console.log(f3(5, 3));
console.log(f3(20, 2147483));
console.log(f3(3, 0.5));
console.log(f3(4, -7.25));
console.log(f3(12, 1000000.0));
console.log(f3(0, 9));
console.log(f3(30, 65536));
