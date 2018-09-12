"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var utils_1 = require("../../../utils");
var FactoryStructurePrinter_1 = require("../../FactoryStructurePrinter");
var SpreadAssignmentStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(SpreadAssignmentStructurePrinter, _super);
    function SpreadAssignmentStructurePrinter() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    SpreadAssignmentStructurePrinter.prototype.printText = function (writer, structure) {
        writer.write("...");
        utils_1.printTextFromStringOrWriter(writer, structure.expression);
    };
    return SpreadAssignmentStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.SpreadAssignmentStructurePrinter = SpreadAssignmentStructurePrinter;
