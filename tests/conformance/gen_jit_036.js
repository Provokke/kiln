function f36(a, b) { let v0 = 2; let v1 = 1; for (let i = 0; i <= a; i = i + 1) { v1 = v0 * a; v0 = v1 + 1; if (v0 > 94075) v0 = v0 % 874; v1 = v0 + b; v1 = v0 - a; if (v1 > 2019) v1 = v1 % 155; v1 = v0 + v1; } return v0 + v1; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f36(k % 7, k);
console.log(acc);
console.log(f36(5, 3));
console.log(f36(20, 2147483));
console.log(f36(3, 0.5));
console.log(f36(4, -7.25));
console.log(f36(12, 1000000.0));
console.log(f36(0, 9));
console.log(f36(30, 65536));
