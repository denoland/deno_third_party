"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
function createHashSet() {
    if (typeof Set !== "undefined")
        return new Set();
    return new Es5HashSet();
}
exports.createHashSet = createHashSet;
var Es5HashSet = /** @class */ (function () {
    function Es5HashSet() {
        this.items = [];
    }
    Es5HashSet.prototype.has = function (value) {
        // slow and O(n)...
        return this.items.indexOf(value) >= 0;
    };
    Es5HashSet.prototype.add = function (value) {
        if (!this.has(value))
            this.items.push(value);
    };
    Es5HashSet.prototype.delete = function (value) {
        var index = this.items.indexOf(value);
        if (index === -1)
            return false;
        this.items.splice(index, 1);
        return true;
    };
    Es5HashSet.prototype.clear = function () {
        this.items.length = 0;
    };
    Es5HashSet.prototype.values = function () {
        var e_1, _a, _b, _c, item, e_1_1;
        return tslib_1.__generator(this, function (_d) {
            switch (_d.label) {
                case 0:
                    _d.trys.push([0, 5, 6, 7]);
                    _b = tslib_1.__values(this.items), _c = _b.next();
                    _d.label = 1;
                case 1:
                    if (!!_c.done) return [3 /*break*/, 4];
                    item = _c.value;
                    return [4 /*yield*/, item];
                case 2:
                    _d.sent();
                    _d.label = 3;
                case 3:
                    _c = _b.next();
                    return [3 /*break*/, 1];
                case 4: return [3 /*break*/, 7];
                case 5:
                    e_1_1 = _d.sent();
                    e_1 = { error: e_1_1 };
                    return [3 /*break*/, 7];
                case 6:
                    try {
                        if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
                    }
                    finally { if (e_1) throw e_1.error; }
                    return [7 /*endfinally*/];
                case 7: return [2 /*return*/];
            }
        });
    };
    return Es5HashSet;
}());
exports.Es5HashSet = Es5HashSet;
