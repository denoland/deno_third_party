"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var Es5WeakMap_1 = require("./Es5WeakMap");
var WeakCache = /** @class */ (function () {
    function WeakCache() {
        if (typeof WeakMap !== undefined)
            this.cacheItems = new WeakMap();
        else
            this.cacheItems = new Es5WeakMap_1.Es5WeakMap();
    }
    WeakCache.prototype.getOrCreate = function (key, createFunc) {
        var item = this.get(key);
        if (item == null) {
            item = createFunc();
            this.set(key, item);
        }
        return item;
    };
    WeakCache.prototype.has = function (key) {
        return this.cacheItems.has(key);
    };
    WeakCache.prototype.get = function (key) {
        return this.cacheItems.get(key);
    };
    WeakCache.prototype.set = function (key, value) {
        this.cacheItems.set(key, value);
    };
    WeakCache.prototype.removeByKey = function (key) {
        this.cacheItems.delete(key);
    };
    return WeakCache;
}());
exports.WeakCache = WeakCache;
