"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var StructurePrinter = /** @class */ (function () {
    function StructurePrinter() {
    }
    // todo: this should not be a method on the base
    StructurePrinter.prototype.printTextOrWriterFunc = function (writer, textOrWriterFunc) {
        if (typeof textOrWriterFunc === "string")
            writer.write(textOrWriterFunc);
        else if (textOrWriterFunc != null)
            textOrWriterFunc(writer);
    };
    return StructurePrinter;
}());
exports.StructurePrinter = StructurePrinter;
