"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var utils_1 = require("../../utils");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var DecoratorStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(DecoratorStructurePrinter, _super);
    function DecoratorStructurePrinter() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    DecoratorStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.printMultiple(writer, structures, function () { return writer.newLine(); });
    };
    DecoratorStructurePrinter.prototype.printTextsInline = function (writer, structures) {
        this.printMultiple(writer, structures, function () { return writer.space(); });
    };
    DecoratorStructurePrinter.prototype.printText = function (writer, structure) {
        writer.write("@" + structure.name);
        this.printArguments(writer, structure);
    };
    DecoratorStructurePrinter.prototype.printArguments = function (writer, structure) {
        if (structure.arguments == null)
            return;
        writer.write("(");
        for (var i = 0; i < structure.arguments.length; i++) {
            writer.conditionalWrite(i > 0, ", ");
            utils_1.printTextFromStringOrWriter(writer, structure.arguments[i]);
        }
        writer.write(")");
    };
    DecoratorStructurePrinter.prototype.printMultiple = function (writer, structures, separator) {
        var e_1, _a;
        if (structures == null || structures.length === 0)
            return;
        try {
            for (var structures_1 = tslib_1.__values(structures), structures_1_1 = structures_1.next(); !structures_1_1.done; structures_1_1 = structures_1.next()) {
                var structure = structures_1_1.value;
                this.printText(writer, structure);
                separator();
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (structures_1_1 && !structures_1_1.done && (_a = structures_1.return)) _a.call(structures_1);
            }
            finally { if (e_1) throw e_1.error; }
        }
    };
    return DecoratorStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.DecoratorStructurePrinter = DecoratorStructurePrinter;
