function f43(a, b) { let v0 = 4; let v1 = 5; let v2 = 3; for (let i = 0; i <= a; i = i + 1) { v2 = b - 8; v2 = v0 - 7; if (v2 > 70015) v2 = v2 % 944; v1 = a - 2; if (v1 > 60422) v1 = v1 % 259; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f43(k % 7, k);
console.log(acc);
console.log(f43(5, 3));
console.log(f43(20, 2147483));
console.log(f43(3, 0.5));
console.log(f43(4, -7.25));
console.log(f43(12, 1000000.0));
console.log(f43(0, 9));
console.log(f43(30, 65536));
