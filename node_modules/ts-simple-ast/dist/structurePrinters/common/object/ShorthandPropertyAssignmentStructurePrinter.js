"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var FactoryStructurePrinter_1 = require("../../FactoryStructurePrinter");
var ShorthandPropertyAssignmentStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(ShorthandPropertyAssignmentStructurePrinter, _super);
    function ShorthandPropertyAssignmentStructurePrinter() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    ShorthandPropertyAssignmentStructurePrinter.prototype.printText = function (writer, structure) {
        writer.write("" + structure.name);
    };
    return ShorthandPropertyAssignmentStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.ShorthandPropertyAssignmentStructurePrinter = ShorthandPropertyAssignmentStructurePrinter;
