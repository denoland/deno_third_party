"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var utils_1 = require("../../../utils");
var FactoryStructurePrinter_1 = require("../../FactoryStructurePrinter");
var PropertyAssignmentStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(PropertyAssignmentStructurePrinter, _super);
    function PropertyAssignmentStructurePrinter() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    PropertyAssignmentStructurePrinter.prototype.printText = function (writer, structure) {
        writer.write(structure.name + ": ");
        utils_1.printTextFromStringOrWriter(writer, structure.initializer);
    };
    return PropertyAssignmentStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.PropertyAssignmentStructurePrinter = PropertyAssignmentStructurePrinter;
