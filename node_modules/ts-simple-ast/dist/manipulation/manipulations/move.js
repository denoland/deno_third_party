"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var nodeHandlers_1 = require("../nodeHandlers");
var textManipulators_1 = require("../textManipulators");
var doManipulation_1 = require("./doManipulation");
/**
 * Changes the child older of two nodes within a parent.
 * @param opts - Options.
 */
function changeChildOrder(opts) {
    var parent = opts.parent, getSiblingFormatting = opts.getSiblingFormatting, oldIndex = opts.oldIndex, newIndex = opts.newIndex;
    doManipulation_1.doManipulation(parent.sourceFile, new textManipulators_1.ChangingChildOrderTextManipulator(opts), new nodeHandlers_1.NodeHandlerFactory().getForChangingChildOrder(opts));
}
exports.changeChildOrder = changeChildOrder;
