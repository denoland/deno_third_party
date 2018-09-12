"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var StructurePrinter_1 = require("../StructurePrinter");
var StringStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(StringStructurePrinter, _super);
    function StringStructurePrinter() {
        return _super.call(this) || this;
    }
    StringStructurePrinter.prototype.printText = function (writer, textOrWriterFunc) {
        if (typeof textOrWriterFunc === "string")
            writer.write(textOrWriterFunc);
        else
            textOrWriterFunc(writer);
    };
    return StringStructurePrinter;
}(StructurePrinter_1.StructurePrinter));
exports.StringStructurePrinter = StringStructurePrinter;
