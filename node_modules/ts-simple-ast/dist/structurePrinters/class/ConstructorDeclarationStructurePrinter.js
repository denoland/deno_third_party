"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var utils_1 = require("../../utils");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var ConstructorDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(ConstructorDeclarationStructurePrinter, _super);
    function ConstructorDeclarationStructurePrinter(factory, options) {
        var _this = _super.call(this, factory) || this;
        _this.options = options;
        return _this;
    }
    ConstructorDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        // todo: move this code to a common printer similar to blank line formatting structure printer
        if (structures == null)
            return;
        for (var i = 0; i < structures.length; i++) {
            if (i > 0) {
                if (this.options.isAmbient)
                    writer.newLine();
                else
                    writer.blankLine();
            }
            this.printText(writer, structures[i]);
        }
    };
    ConstructorDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        var _this = this;
        this.printOverloads(writer, getOverloadStructures());
        this.printBase(writer, structure);
        if (this.options.isAmbient)
            writer.write(";");
        else
            writer.space().inlineBlock(function () {
                _this.factory.forBodyText(_this.options).printText(writer, structure);
            });
        function getOverloadStructures() {
            var e_1, _a;
            // all the overloads need to have the same scope as the implementation
            var overloads = utils_1.ObjectUtils.clone(structure.overloads);
            if (overloads == null || overloads.length === 0)
                return;
            try {
                for (var overloads_1 = tslib_1.__values(overloads), overloads_1_1 = overloads_1.next(); !overloads_1_1.done; overloads_1_1 = overloads_1.next()) {
                    var overload = overloads_1_1.value;
                    utils_1.setValueIfUndefined(overload, "scope", structure.scope);
                }
            }
            catch (e_1_1) { e_1 = { error: e_1_1 }; }
            finally {
                try {
                    if (overloads_1_1 && !overloads_1_1.done && (_a = overloads_1.return)) _a.call(overloads_1);
                }
                finally { if (e_1) throw e_1.error; }
            }
            return overloads;
        }
    };
    ConstructorDeclarationStructurePrinter.prototype.printOverloads = function (writer, structures) {
        var e_2, _a;
        if (structures == null || structures.length === 0)
            return;
        try {
            for (var structures_1 = tslib_1.__values(structures), structures_1_1 = structures_1.next(); !structures_1_1.done; structures_1_1 = structures_1.next()) {
                var structure = structures_1_1.value;
                this.printOverload(writer, structure);
                writer.newLine();
            }
        }
        catch (e_2_1) { e_2 = { error: e_2_1 }; }
        finally {
            try {
                if (structures_1_1 && !structures_1_1.done && (_a = structures_1.return)) _a.call(structures_1);
            }
            finally { if (e_2) throw e_2.error; }
        }
    };
    ConstructorDeclarationStructurePrinter.prototype.printOverload = function (writer, structure) {
        this.printBase(writer, structure);
        writer.write(";");
    };
    ConstructorDeclarationStructurePrinter.prototype.printBase = function (writer, structure) {
        this.factory.forJSDoc().printDocs(writer, structure.docs);
        this.factory.forModifierableNode().printText(writer, structure);
        writer.write("constructor");
        this.factory.forTypeParameterDeclaration().printTextsWithBrackets(writer, structure.typeParameters);
        writer.write("(");
        this.factory.forParameterDeclaration().printTexts(writer, structure.parameters);
        writer.write(")");
    };
    return ConstructorDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.ConstructorDeclarationStructurePrinter = ConstructorDeclarationStructurePrinter;
