function f54(a, b) { let v0 = 4; let v1 = 2; let v2 = 2; for (let i = 0; i !== a; i = i + 1) { v1 = 4 + v1; if (v1 > 61912) v1 = v1 % 669; v1 = v2 * v2; } return v0 + v1 + v2; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f54(k % 7, k);
console.log(acc);
console.log(f54(5, 3));
console.log(f54(20, 2147483));
console.log(f54(3, 0.5));
console.log(f54(4, -7.25));
console.log(f54(12, 1000000.0));
console.log(f54(0, 9));
console.log(f54(30, 65536));
