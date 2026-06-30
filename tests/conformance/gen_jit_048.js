function f48(a, b) { let v0 = 0; for (let i = 0; i <= a; i = i + 1) { v0 = 8 + 8; if (v0 > 36643) v0 = v0 % 418; v0 = 8 + v0; } return v0; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f48(k % 7, k);
console.log(acc);
console.log(f48(5, 3));
console.log(f48(20, 2147483));
console.log(f48(3, 0.5));
console.log(f48(4, -7.25));
console.log(f48(12, 1000000.0));
console.log(f48(0, 9));
console.log(f48(30, 65536));
