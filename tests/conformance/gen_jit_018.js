function f18(a, b) { let v0 = 1; let v1 = 3; let v2 = 5; for (let i = 0; i < a; i = i + 1) { v0 = v0 - a; if (v0 > 99303) v0 = v0 % 630; v2 = v1 + a; v1 = 1 * a; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f18(k % 7, k);
console.log(acc);
console.log(f18(5, 3));
console.log(f18(20, 2147483));
console.log(f18(3, 0.5));
console.log(f18(4, -7.25));
console.log(f18(12, 1000000.0));
console.log(f18(0, 9));
console.log(f18(30, 65536));
