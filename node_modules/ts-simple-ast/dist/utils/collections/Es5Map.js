"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Es5PropSaver_1 = require("../Es5PropSaver");
var Es5Map = /** @class */ (function () {
    function Es5Map() {
        this.propSaver = new Es5PropSaver_1.Es5PropSaver();
        this.items = {};
        this.itemCount = 0;
    }
    Object.defineProperty(Es5Map.prototype, "size", {
        get: function () {
            return Object.keys(this.items).length;
        },
        enumerable: true,
        configurable: true
    });
    Es5Map.prototype.set = function (key, value) {
        var identifier = this.getIdentifier(key) || this.createIdentifier(key);
        this.items[identifier] = [key, value];
    };
    Es5Map.prototype.get = function (key) {
        var identifier = this.getIdentifier(key);
        if (identifier == null)
            return undefined;
        var keyValue = this.items[identifier];
        if (keyValue == null)
            return undefined;
        return keyValue[1];
    };
    Es5Map.prototype.has = function (key) {
        var identifier = this.getIdentifier(key);
        if (identifier == null)
            return false;
        return this.items.hasOwnProperty(identifier);
    };
    Es5Map.prototype.delete = function (key) {
        var identifier = this.getIdentifier(key);
        if (identifier != null)
            delete this.items[identifier];
    };
    Es5Map.prototype.clear = function () {
        this.propSaver = new Es5PropSaver_1.Es5PropSaver();
        this.items = {};
    };
    Es5Map.prototype.entries = function () {
        var e_1, _a, _b, _c, key, e_1_1;
        return tslib_1.__generator(this, function (_d) {
            switch (_d.label) {
                case 0:
                    _d.trys.push([0, 5, 6, 7]);
                    _b = tslib_1.__values(Object.keys(this.items)), _c = _b.next();
                    _d.label = 1;
                case 1:
                    if (!!_c.done) return [3 /*break*/, 4];
                    key = _c.value;
                    return [4 /*yield*/, this.items[key]];
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
    Es5Map.prototype.keys = function () {
        var e_2, _a, _b, _c, entry, e_2_1;
        return tslib_1.__generator(this, function (_d) {
            switch (_d.label) {
                case 0:
                    _d.trys.push([0, 5, 6, 7]);
                    _b = tslib_1.__values(this.entries()), _c = _b.next();
                    _d.label = 1;
                case 1:
                    if (!!_c.done) return [3 /*break*/, 4];
                    entry = _c.value;
                    return [4 /*yield*/, entry[0]];
                case 2:
                    _d.sent();
                    _d.label = 3;
                case 3:
                    _c = _b.next();
                    return [3 /*break*/, 1];
                case 4: return [3 /*break*/, 7];
                case 5:
                    e_2_1 = _d.sent();
                    e_2 = { error: e_2_1 };
                    return [3 /*break*/, 7];
                case 6:
                    try {
                        if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
                    }
                    finally { if (e_2) throw e_2.error; }
                    return [7 /*endfinally*/];
                case 7: return [2 /*return*/];
            }
        });
    };
    Es5Map.prototype.values = function () {
        var e_3, _a, _b, _c, entry, e_3_1;
        return tslib_1.__generator(this, function (_d) {
            switch (_d.label) {
                case 0:
                    _d.trys.push([0, 5, 6, 7]);
                    _b = tslib_1.__values(this.entries()), _c = _b.next();
                    _d.label = 1;
                case 1:
                    if (!!_c.done) return [3 /*break*/, 4];
                    entry = _c.value;
                    return [4 /*yield*/, entry[1]];
                case 2:
                    _d.sent();
                    _d.label = 3;
                case 3:
                    _c = _b.next();
                    return [3 /*break*/, 1];
                case 4: return [3 /*break*/, 7];
                case 5:
                    e_3_1 = _d.sent();
                    e_3 = { error: e_3_1 };
                    return [3 /*break*/, 7];
                case 6:
                    try {
                        if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
                    }
                    finally { if (e_3) throw e_3.error; }
                    return [7 /*endfinally*/];
                case 7: return [2 /*return*/];
            }
        });
    };
    Es5Map.prototype.getIdentifier = function (key) {
        if (typeof key === "string")
            return key;
        return this.propSaver.get(key);
    };
    Es5Map.prototype.createIdentifier = function (key) {
        if (typeof key === "string")
            return key;
        var identifier = (this.itemCount++).toString();
        this.propSaver.set(key, identifier);
        return identifier;
    };
    return Es5Map;
}());
exports.Es5Map = Es5Map;
