"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var StructurePrinter_1 = require("../StructurePrinter");
var NewLineFormattingStructuresPrinter = /** @class */ (function (_super) {
    tslib_1.__extends(NewLineFormattingStructuresPrinter, _super);
    function NewLineFormattingStructuresPrinter(structurePrinter) {
        var _this = _super.call(this) || this;
        _this.structurePrinter = structurePrinter;
        return _this;
    }
    NewLineFormattingStructuresPrinter.prototype.printText = function (writer, structures) {
        if (structures == null)
            return;
        for (var i = 0; i < structures.length; i++) {
            writer.conditionalNewLine(i > 0);
            this.structurePrinter.printText(writer, structures[i]);
        }
    };
    return NewLineFormattingStructuresPrinter;
}(StructurePrinter_1.StructurePrinter));
exports.NewLineFormattingStructuresPrinter = NewLineFormattingStructuresPrinter;
