"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var utils_1 = require("../../utils");
var FactoryStructurePrinter_1 = require("../FactoryStructurePrinter");
var formatting_1 = require("../formatting");
var ClassDeclarationStructurePrinter = /** @class */ (function (_super) {
    tslib_1.__extends(ClassDeclarationStructurePrinter, _super);
    function ClassDeclarationStructurePrinter(factory, options) {
        var _this = _super.call(this, factory) || this;
        _this.options = options;
        _this.multipleWriter = new formatting_1.BlankLineFormattingStructuresPrinter(_this);
        return _this;
    }
    ClassDeclarationStructurePrinter.prototype.printTexts = function (writer, structures) {
        this.multipleWriter.printText(writer, structures);
    };
    ClassDeclarationStructurePrinter.prototype.printText = function (writer, structure) {
        var _this = this;
        var isAmbient = structure.hasDeclareKeyword || this.options.isAmbient;
        this.factory.forJSDoc().printDocs(writer, structure.docs);
        this.factory.forDecorator().printTexts(writer, structure.decorators);
        this.factory.forModifierableNode().printText(writer, structure);
        writer.write("class");
        // can be null, ex. `export default class { ... }`
        if (!utils_1.StringUtils.isNullOrWhitespace(structure.name))
            writer.space().write(structure.name);
        this.factory.forTypeParameterDeclaration().printTextsWithBrackets(writer, structure.typeParameters);
        writer.space();
        if (!utils_1.StringUtils.isNullOrWhitespace(structure.extends))
            writer.write("extends " + structure.extends + " ");
        if (!utils_1.ArrayUtils.isNullOrEmpty(structure.implements))
            writer.write("implements " + structure.implements.join(", ") + " ");
        writer.inlineBlock(function () {
            _this.factory.forPropertyDeclaration().printTexts(writer, structure.properties);
            _this.printCtors(writer, structure, isAmbient);
            _this.printGetAndSet(writer, structure, isAmbient);
            if (!utils_1.ArrayUtils.isNullOrEmpty(structure.methods)) {
                _this.conditionalSeparator(writer, isAmbient);
                _this.factory.forMethodDeclaration({ isAmbient: isAmbient }).printTexts(writer, structure.methods);
            }
        });
    };
    ClassDeclarationStructurePrinter.prototype.printCtors = function (writer, structure, isAmbient) {
        var e_1, _a;
        if (utils_1.ArrayUtils.isNullOrEmpty(structure.ctors))
            return;
        try {
            for (var _b = tslib_1.__values(structure.ctors), _c = _b.next(); !_c.done; _c = _b.next()) {
                var ctor = _c.value;
                this.conditionalSeparator(writer, isAmbient);
                this.factory.forConstructorDeclaration({ isAmbient: isAmbient }).printText(writer, ctor);
            }
        }
        catch (e_1_1) { e_1 = { error: e_1_1 }; }
        finally {
            try {
                if (_c && !_c.done && (_a = _b.return)) _a.call(_b);
            }
            finally { if (e_1) throw e_1.error; }
        }
    };
    ClassDeclarationStructurePrinter.prototype.printGetAndSet = function (writer, structure, isAmbient) {
        var e_2, _a, e_3, _b;
        var getAccessors = tslib_1.__spread(structure.getAccessors || []);
        var setAccessors = tslib_1.__spread(structure.setAccessors || []);
        var getAccessorWriter = this.factory.forGetAccessorDeclaration({ isAmbient: isAmbient });
        var setAccessorWriter = this.factory.forSetAccessorDeclaration({ isAmbient: isAmbient });
        var _loop_1 = function (getAccessor) {
            this_1.conditionalSeparator(writer, isAmbient);
            getAccessorWriter.printText(writer, getAccessor);
            // write the corresponding set accessor beside the get accessor
            var setAccessorIndex = utils_1.ArrayUtils.findIndex(setAccessors, function (item) { return item.name === getAccessor.name; });
            if (setAccessorIndex >= 0) {
                this_1.conditionalSeparator(writer, isAmbient);
                setAccessorWriter.printText(writer, setAccessors[setAccessorIndex]);
                setAccessors.splice(setAccessorIndex, 1);
            }
        };
        var this_1 = this;
        try {
            for (var getAccessors_1 = tslib_1.__values(getAccessors), getAccessors_1_1 = getAccessors_1.next(); !getAccessors_1_1.done; getAccessors_1_1 = getAccessors_1.next()) {
                var getAccessor = getAccessors_1_1.value;
                _loop_1(getAccessor);
            }
        }
        catch (e_2_1) { e_2 = { error: e_2_1 }; }
        finally {
            try {
                if (getAccessors_1_1 && !getAccessors_1_1.done && (_a = getAccessors_1.return)) _a.call(getAccessors_1);
            }
            finally { if (e_2) throw e_2.error; }
        }
        try {
            for (var setAccessors_1 = tslib_1.__values(setAccessors), setAccessors_1_1 = setAccessors_1.next(); !setAccessors_1_1.done; setAccessors_1_1 = setAccessors_1.next()) {
                var setAccessor = setAccessors_1_1.value;
                this.conditionalSeparator(writer, isAmbient);
                setAccessorWriter.printText(writer, setAccessor);
            }
        }
        catch (e_3_1) { e_3 = { error: e_3_1 }; }
        finally {
            try {
                if (setAccessors_1_1 && !setAccessors_1_1.done && (_b = setAccessors_1.return)) _b.call(setAccessors_1);
            }
            finally { if (e_3) throw e_3.error; }
        }
    };
    ClassDeclarationStructurePrinter.prototype.conditionalSeparator = function (writer, isAmbient) {
        if (writer.isAtStartOfFirstLineOfBlock())
            return;
        if (isAmbient)
            writer.newLine();
        else
            writer.blankLine();
    };
    return ClassDeclarationStructurePrinter;
}(FactoryStructurePrinter_1.FactoryStructurePrinter));
exports.ClassDeclarationStructurePrinter = ClassDeclarationStructurePrinter;
