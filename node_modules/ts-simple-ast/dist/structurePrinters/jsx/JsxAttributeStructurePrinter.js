"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var JsxAttributeStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(JsxAttributeStructurePrinter, _super);
    function JsxAttributeStructurePrinter() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    JsxAttributeStructurePrinter.prototype.printText = function (writer, structure) {
        if (structure.isSpreadAttribute)
            writer.write("...");
        writer.write(structure.name);
        if (structure.initializer != null)
            writer.write("=").write(structure.initializer);
    };
    return JsxAttributeStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.JsxAttributeStructurePrinter = JsxAttributeStructurePrinter;
