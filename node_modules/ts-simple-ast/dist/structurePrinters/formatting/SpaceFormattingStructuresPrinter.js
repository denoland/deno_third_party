"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var StructurePrinter_1 = require("../StructurePrinter");
var SpaceFormattingStructuresPrinter = /** @class */ (function (_super) {
    tslib_1.__extends(SpaceFormattingStructuresPrinter, _super);
    function SpaceFormattingStructuresPrinter(structurePrinter) {
        var _this = _super.call(this) || this;
        _this.structurePrinter = structurePrinter;
        return _this;
    }
    SpaceFormattingStructuresPrinter.prototype.printText = function (writer, structures) {
        for (var i = 0; i < structures.length; i++) {
            writer.conditionalWrite(i > 0, " ");
            this.structurePrinter.printText(writer, structures[i]);
        }
    };
    return SpaceFormattingStructuresPrinter;
}(StructurePrinter_1.StructurePrinter));
exports.SpaceFormattingStructuresPrinter = SpaceFormattingStructuresPrinter;
