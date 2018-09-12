"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var StructurePrinter_1 = require("./StructurePrinter");
var FactoryStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(FactoryStructurePrinter, _super);
    function FactoryStructurePrinter(factory) {
        var _this = _super.call(this) || this;
        _this.factory = factory;
        return _this;
    }
    return FactoryStructurePrinter;
}(StructurePrinter_1.StructurePrinter));
exports.FactoryStructurePrinter = FactoryStructurePrinter;
