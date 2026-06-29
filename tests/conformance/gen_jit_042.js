function f42(a, b) { let v0 = 3; let v1 = 3; for (let i = 0; i <= a; i = i + 1) { v0 = 2 + a; if (v0 > 83895) v0 = v0 % 643; v0 = b * a; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f42(k % 7, k);
console.log(acc);
console.log(f42(5, 3));
console.log(f42(20, 2147483));
console.log(f42(3, 0.5));
console.log(f42(4, -7.25));
console.log(f42(12, 1000000.0));
console.log(f42(0, 9));
console.log(f42(30, 65536));
