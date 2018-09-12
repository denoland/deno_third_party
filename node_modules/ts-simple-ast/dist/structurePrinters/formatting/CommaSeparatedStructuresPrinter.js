"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var StructurePrinter_1 = require("../StructurePrinter");
var CommaSeparatedStructuresPrinter = /** @class */ (function (_super) {
    tslib_1.__extends(CommaSeparatedStructuresPrinter, _super);
    function CommaSeparatedStructuresPrinter(structurePrinter) {
        var _this = _super.call(this) || this;
        _this.structurePrinter = structurePrinter;
        return _this;
    }
    CommaSeparatedStructuresPrinter.prototype.printText = function (writer, structures) {
        if (structures == null)
            return;
        for (var i = 0; i < structures.length; i++) {
            if (i > 0)
                writer.write(", ");
            this.structurePrinter.printText(writer, structures[i]);
        }
    };
    return CommaSeparatedStructuresPrinter;
}(StructurePrinter_1.StructurePrinter));
exports.CommaSeparatedStructuresPrinter = CommaSeparatedStructuresPrinter;
