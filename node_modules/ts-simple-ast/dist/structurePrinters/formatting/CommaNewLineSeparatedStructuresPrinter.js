"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var StructurePrinter_1 = require("../StructurePrinter");
var CommaNewLineSeparatedStructuresPrinter = /** @class */ (function (_super) {
    tslib_1.__extends(CommaNewLineSeparatedStructuresPrinter, _super);
    function CommaNewLineSeparatedStructuresPrinter(structurePrinter) {
        var _this = _super.call(this) || this;
        _this.structurePrinter = structurePrinter;
        return _this;
    }
    CommaNewLineSeparatedStructuresPrinter.prototype.printText = function (writer, structures) {
        if (structures == null)
            return;
        for (var i = 0; i < structures.length; i++) {
            if (i > 0)
                writer.write(",").newLine();
            this.structurePrinter.printText(writer, structures[i]);
        }
    };
    return CommaNewLineSeparatedStructuresPrinter;
}(StructurePrinter_1.StructurePrinter));
exports.CommaNewLineSeparatedStructuresPrinter = CommaNewLineSeparatedStructuresPrinter;
