"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var errors = require("../../errors");
var manipulation_1 = require("../../manipulation");
var typescript_1 = require("../../typescript");
var utils_1 = require("../../utils");
function ArgumentedNode(Base) {
    return /** @class */ (function (_super) {
        tslib_1.__extends(class_1, _super);
        function class_1() {
            return _super !== null && _super.apply(this, arguments) || this;
        }
        class_1.prototype.getArguments = function () {
            var _this = this;
            return this.compilerNode.arguments.map(function (a) { return _this.getNodeFromCompilerNode(a); });
        };
        class_1.prototype.addArgument = function (argumentText) {
            return this.addArguments([argumentText])[0];
        };
        class_1.prototype.addArguments = function (argumentTexts) {
            return this.insertArguments(this.getArguments().length, argumentTexts);
        };
        class_1.prototype.insertArgument = function (index, argumentText) {
            return this.insertArguments(index, [argumentText])[0];
        };
        class_1.prototype.insertArguments = function (index, argumentTexts) {
            if (utils_1.ArrayUtils.isNullOrEmpty(argumentTexts))
                return [];
            var args = this.getArguments();
            index = manipulation_1.verifyAndGetIndex(index, args.length);
            var writer = this.getWriterWithQueuedChildIndentation();
            for (var i = 0; i < argumentTexts.length; i++) {
                writer.conditionalWrite(i > 0, ", ");
                utils_1.printTextFromStringOrWriter(writer, argumentTexts[i]);
            }
            manipulation_1.insertIntoCommaSeparatedNodes({
                parent: this.getFirstChildByKindOrThrow(typescript_1.SyntaxKind.OpenParenToken).getNextSiblingIfKindOrThrow(typescript_1.SyntaxKind.SyntaxList),
                currentNodes: args,
                insertIndex: index,
                newText: writer.toString()
            });
            return manipulation_1.getNodesToReturn(this.getArguments(), index, argumentTexts.length);
        };
        class_1.prototype.removeArgument = function (argOrIndex) {
            var args = this.getArguments();
            if (args.length === 0)
                throw new errors.InvalidOperationError("Cannot remove an argument when none exist.");
            var argToRemove = typeof argOrIndex === "number" ? getArgFromIndex(argOrIndex) : argOrIndex;
            manipulation_1.removeCommaSeparatedChild(argToRemove);
            return this;
            function getArgFromIndex(index) {
                return args[manipulation_1.verifyAndGetIndex(index, args.length - 1)];
            }
        };
        return class_1;
    }(Base));
}
exports.ArgumentedNode = ArgumentedNode;
