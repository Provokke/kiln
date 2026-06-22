function f27(a, b) { let v0 = 5; let v1 = 0; let v2 = 3; for (let i = 0; i <= a; i = i + 1) { v1 = v0 * b; v1 = v1 - 3; if (v1 > 42448) v1 = v1 % 244; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f27(k % 7, k);
console.log(acc);
console.log(f27(5, 3));
console.log(f27(20, 2147483));
console.log(f27(3, 0.5));
console.log(f27(4, -7.25));
console.log(f27(12, 1000000.0));
console.log(f27(0, 9));
console.log(f27(30, 65536));
