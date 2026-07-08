function f56(a, b) { let v0 = 4; for (let i = 0; i !== a; i = i + 1) { v0 = v0 - a; v0 = 7 + 1; if (v0 > 92675) v0 = v0 % 565; v0 = a - 6; v0 = a + b; v0 = b + 5; } return v0; }
let acc = 0; for (let k = 0; k < 60; k++) acc = acc + f56(k % 7, k);
console.log(acc);
console.log(f56(5, 3));
console.log(f56(20, 2147483));
console.log(f56(3, 0.5));
console.log(f56(4, -7.25));
console.log(f56(12, 1000000.0));
console.log(f56(0, 9));
console.log(f56(30, 65536));
